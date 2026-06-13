use crate::atomic::append_crc32c;
use crate::error::StorageError;
use crate::format::{LEGACY_SEGMENT_MAGIC, LEGACY_SEGMENT_V2_MAGIC};
use crate::wal::checksum::crc32c;

use super::{
    SegmentCandidateEntry, SegmentCell, SegmentCellRef, SegmentDescriptorEntry, SegmentReader,
    SegmentWriter,
};

fn sample_cells() -> Vec<SegmentCell> {
    vec![
        SegmentCell {
            candidate_id: 1,
            cell_id: 10,
            created_seq: 1,
            deleted_seq: None,
            payload: b"first payload body".to_vec(),
        },
        SegmentCell {
            candidate_id: 2,
            cell_id: 20,
            created_seq: 2,
            deleted_seq: Some(5),
            payload: b"second, tombstoned".to_vec(),
        },
    ]
}

#[test]
fn read_candidate_entries_matches_full_read_without_payload() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("segment.acs");
    SegmentWriter::write(&path, &sample_cells()).unwrap();

    let full = SegmentReader::read(&path).unwrap();
    let entries = SegmentReader::read_candidate_entries(&path).unwrap();

    let expected: Vec<SegmentCandidateEntry> = full
        .iter()
        .map(|cell| SegmentCandidateEntry {
            candidate_id: cell.candidate_id,
            cell_id: cell.cell_id,
            deleted: cell.deleted_seq.is_some(),
        })
        .collect();
    assert_eq!(entries, expected);
}

#[test]
fn segment_records_roundtrip_optional_descriptor_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("segment.acs");
    let payload = b"typed payload".to_vec();
    let descriptor = b"descriptor bytes".to_vec();
    let cells = [SegmentCellRef {
        candidate_id: 7,
        cell_id: 70,
        created_seq: 3,
        deleted_seq: None,
        descriptor: Some(descriptor.clone()),
        payload: &payload,
    }];

    SegmentWriter::write_refs(&path, &cells).unwrap();

    assert_eq!(
        SegmentReader::read(&path).unwrap(),
        vec![SegmentCell {
            candidate_id: 7,
            cell_id: 70,
            created_seq: 3,
            deleted_seq: None,
            payload,
        }]
    );
    let records = SegmentReader::read_records(&path).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].descriptor.as_deref(),
        Some(descriptor.as_slice())
    );
}

#[test]
fn read_descriptors_and_payload_at_use_footer_offsets() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("segment.acs");
    let payload = b"footer backed payload".to_vec();
    let descriptor = b"descriptor bytes".to_vec();
    let cells = [SegmentCellRef {
        candidate_id: 7,
        cell_id: 70,
        created_seq: 3,
        deleted_seq: None,
        descriptor: Some(descriptor.clone()),
        payload: &payload,
    }];

    SegmentWriter::write_refs(&path, &cells).unwrap();

    let descriptors = SegmentReader::read_descriptors(&path).unwrap();
    assert_eq!(
        descriptors,
        vec![SegmentDescriptorEntry {
            candidate_id: 7,
            cell_id: 70,
            created_seq: 3,
            deleted_seq: None,
            descriptor: Some(descriptor),
            payload_offset: 8,
            payload_len: payload.len() as u64,
            payload_crc32c: crc32c(&payload),
        }]
    );
    assert_eq!(
        SegmentReader::read_payload_at(&path, 7).unwrap(),
        Some(payload)
    );
    assert_eq!(SegmentReader::read_payload_at(&path, 8).unwrap(), None);
}

#[test]
fn read_payload_at_does_not_decode_unrelated_payload_blocks() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("segment.acs");
    SegmentWriter::write(&path, &sample_cells()).unwrap();
    let descriptors = SegmentReader::read_descriptors(&path).unwrap();
    let unrelated = descriptors
        .iter()
        .find(|entry| entry.candidate_id == 2)
        .unwrap();

    let mut bytes = std::fs::read(&path).unwrap();
    bytes[unrelated.payload_offset as usize] ^= 0xff;
    std::fs::write(&path, bytes).unwrap();

    assert_eq!(
        SegmentReader::read_payload_at(&path, 1).unwrap(),
        Some(b"first payload body".to_vec())
    );
    assert!(matches!(
        SegmentReader::read(&path),
        Err(StorageError::InvalidSegmentFile)
    ));
}

#[test]
fn read_payload_at_detects_corrupt_target_payload_block() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("segment.acs");
    SegmentWriter::write(&path, &sample_cells()).unwrap();
    let descriptors = SegmentReader::read_descriptors(&path).unwrap();
    let target = descriptors
        .iter()
        .find(|entry| entry.candidate_id == 1)
        .unwrap();

    let mut bytes = std::fs::read(&path).unwrap();
    bytes[target.payload_offset as usize] ^= 0xff;
    rewrite_file_crc(&mut bytes);
    std::fs::write(&path, bytes).unwrap();

    assert!(matches!(
        SegmentReader::read_payload_at(&path, 1),
        Err(StorageError::InvalidSegmentFile)
    ));
}

#[test]
fn legacy_acs1_segments_remain_readable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("segment.acs");
    write_legacy_acs1(&path, &sample_cells());

    assert_eq!(SegmentReader::read(&path).unwrap(), sample_cells());
    assert_eq!(
        SegmentReader::read_payload_at(&path, 2).unwrap(),
        Some(b"second, tombstoned".to_vec())
    );
    let descriptors = SegmentReader::read_descriptors(&path).unwrap();
    assert_eq!(descriptors.len(), 2);
    assert!(descriptors.iter().all(|entry| entry.descriptor.is_none()));
}

#[test]
fn old_linear_acs2_segments_remain_readable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("segment.acs");
    let payload = b"old linear payload".to_vec();
    let descriptor = b"old descriptor".to_vec();
    let cells = [SegmentCellRef {
        candidate_id: 9,
        cell_id: 90,
        created_seq: 4,
        deleted_seq: None,
        descriptor: Some(descriptor.clone()),
        payload: &payload,
    }];
    write_linear_acs2(&path, &cells);

    assert_eq!(
        SegmentReader::read_payload_at(&path, 9).unwrap(),
        Some(payload.clone())
    );
    assert_eq!(
        SegmentReader::read_records(&path).unwrap()[0]
            .descriptor
            .as_deref(),
        Some(descriptor.as_slice())
    );
    assert_eq!(
        SegmentReader::read_descriptors(&path).unwrap()[0]
            .descriptor
            .as_deref(),
        Some(descriptor.as_slice())
    );
}

fn write_legacy_acs1(path: &std::path::Path, cells: &[SegmentCell]) {
    let mut out = Vec::new();
    out.extend_from_slice(&LEGACY_SEGMENT_MAGIC);
    put_u32(&mut out, cells.len() as u32);
    for cell in cells {
        put_u64(&mut out, cell.cell_id);
        put_u32(&mut out, cell.candidate_id);
        put_u64(&mut out, cell.created_seq);
        put_u64(&mut out, cell.deleted_seq.unwrap_or(0));
        put_u32(&mut out, cell.payload.len() as u32);
        out.extend_from_slice(&cell.payload);
    }
    append_crc32c(&mut out);
    std::fs::write(path, out).unwrap();
}

fn write_linear_acs2(path: &std::path::Path, cells: &[SegmentCellRef<'_>]) {
    let mut out = Vec::new();
    out.extend_from_slice(&LEGACY_SEGMENT_V2_MAGIC);
    put_u32(&mut out, cells.len() as u32);
    for cell in cells {
        put_u64(&mut out, cell.cell_id);
        put_u32(&mut out, cell.candidate_id);
        put_u64(&mut out, cell.created_seq);
        put_u64(&mut out, cell.deleted_seq.unwrap_or(0));
        let descriptor = cell.descriptor.as_deref().unwrap_or(&[]);
        put_u32(&mut out, descriptor.len() as u32);
        out.extend_from_slice(descriptor);
        put_u32(&mut out, cell.payload.len() as u32);
        out.extend_from_slice(cell.payload);
    }
    append_crc32c(&mut out);
    std::fs::write(path, out).unwrap();
}

fn rewrite_file_crc(bytes: &mut [u8]) {
    let crc_offset = bytes.len() - 4;
    let checksum = crc32c(&bytes[..crc_offset]);
    bytes[crc_offset..].copy_from_slice(&checksum.to_le_bytes());
}

fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}
