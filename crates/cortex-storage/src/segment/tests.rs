use super::{SegmentCandidateEntry, SegmentCell, SegmentCellRef, SegmentReader, SegmentWriter};

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
