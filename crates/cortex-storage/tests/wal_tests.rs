use std::fs::File;
use std::io::Write;

use cortex_storage::wal::{
    DurabilityMode, SectionTag, WalCodec, WalDiagnostics, WalReader, WalRecord, WalRecordType,
    WalSection, WalWriter, WalWriterOptions, WAL_RECORD_HEADER_LEN, WAL_SECTION_ENTRY_LEN,
};
use cortex_storage::StorageError;

fn record_with_payload(payload: Vec<u8>) -> WalRecord {
    WalRecord::new(
        WalRecordType::PutCellBatch,
        vec![WalSection::new(SectionTag::PayloadInline, payload)],
    )
}

fn record() -> WalRecord {
    WalRecord::new(
        WalRecordType::PutCellBatch,
        vec![
            WalSection::new(SectionTag::CellCore, b"cell-core".to_vec()),
            WalSection::new(SectionTag::PayloadInline, b"payload".to_vec()),
        ],
    )
}

#[test]
fn empty_payload_roundtrip() {
    let record = WalRecord::new(WalRecordType::Checkpoint, Vec::new());
    let encoded = WalCodec::encode_record(&record).unwrap();
    let decoded = WalCodec::decode_record(&encoded).unwrap();
    assert_eq!(decoded.record, record);
    assert_eq!(decoded.header.payload_len, 0);
}

#[test]
fn one_section_roundtrip() {
    let record = record_with_payload(b"payload".to_vec());
    let encoded = WalCodec::encode_record_at(&record, 42).unwrap();
    let decoded = WalCodec::decode_record(&encoded).unwrap();
    assert_eq!(decoded.lsn, 42);
    assert_eq!(decoded.record, record);
    assert_eq!(decoded.sections.len(), 1);
    assert_eq!(decoded.bytes_consumed, encoded.len());
}

#[test]
fn corrupt_payload_is_rejected() {
    let mut encoded = WalCodec::encode_record_at(&record(), 42).unwrap();
    *encoded.last_mut().unwrap() ^= 0xff;
    assert!(matches!(
        WalCodec::decode_record(&encoded).unwrap_err(),
        StorageError::WalChecksumMismatch
    ));
}

#[test]
fn corrupt_header_is_invalid_record() {
    let mut encoded = WalCodec::encode_record_at(&record(), 42).unwrap();
    encoded[4] ^= 0xff;
    assert!(matches!(
        WalCodec::decode_record(&encoded).unwrap_err(),
        StorageError::InvalidWalRecord
    ));
}

#[test]
fn partial_tail_is_reported() {
    let mut encoded = WalCodec::encode_record_at(&record(), 42).unwrap();
    encoded.truncate(encoded.len() - 3);
    assert!(matches!(
        WalCodec::decode_record(&encoded).unwrap_err(),
        StorageError::IncompleteTail
    ));
}

#[test]
fn unknown_section_tag_is_preserved_and_skipped_from_known_record() {
    let mut encoded = WalCodec::encode_record_at(&record(), 42).unwrap();
    encoded[WAL_RECORD_HEADER_LEN..WAL_RECORD_HEADER_LEN + 2]
        .copy_from_slice(&999u16.to_le_bytes());
    rewrite_header_crc(&mut encoded);
    let decoded = WalCodec::decode_record(&encoded).unwrap();
    assert_eq!(decoded.sections[0].tag, None);
    assert_eq!(decoded.sections[0].tag_raw, 999);
    assert_eq!(decoded.record.sections.len(), 1);
    assert_eq!(decoded.record.sections[0].tag, SectionTag::PayloadInline);
}

#[test]
fn sections_are_8_byte_aligned_with_zero_padding() {
    let record = WalRecord::new(
        WalRecordType::PutCellBatch,
        vec![
            WalSection::new(SectionTag::CellCore, vec![1]),
            WalSection::new(SectionTag::PayloadInline, vec![2, 3]),
        ],
    );
    let encoded = WalCodec::encode_record(&record).unwrap();
    let second_entry = WAL_RECORD_HEADER_LEN + WAL_SECTION_ENTRY_LEN;
    let second_offset = get_u32(&encoded[second_entry + 4..second_entry + 8]);
    let header_len = get_u16(&encoded[4..6]) as usize;
    assert_eq!(second_offset, 8);
    assert_eq!(&encoded[header_len + 1..header_len + 8], &[0; 7]);
}

#[test]
fn payload_size_matrix_roundtrips() {
    for size in [0usize, 1, 7, 8, 9, 1024, 4096] {
        let payload = (0..size).map(|index| index as u8).collect::<Vec<_>>();
        let record = record_with_payload(payload);
        let encoded = WalCodec::encode_record(&record).unwrap();
        let decoded = WalCodec::decode_record(&encoded).unwrap();
        assert_eq!(decoded.record, record);
    }
}

#[test]
fn reader_scans_multiple_records() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("multi.aclog");
    let first = WalCodec::encode_record_at(&record(), 16).unwrap();
    let second =
        WalCodec::encode_record_at(&record_with_payload(vec![9]), 16 + first.len() as u64).unwrap();
    let mut file = File::create(&path).unwrap();
    file.write_all(&WalCodec::file_header()).unwrap();
    file.write_all(&first).unwrap();
    file.write_all(&second).unwrap();
    drop(file);
    let scan = WalReader::open(&path).unwrap().scan().unwrap();
    assert_eq!(scan.records.len(), 2);
    assert_eq!(
        scan.safe_truncate_offset,
        (16 + first.len() + second.len()) as u64
    );
}

#[test]
fn diagnostics_summarize_scan_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("diag.aclog");
    let first = WalCodec::encode_record_at(&record(), 16).unwrap();
    let second =
        WalCodec::encode_record_at(&record_with_payload(vec![9]), 16 + first.len() as u64).unwrap();
    let mut file = File::create(&path).unwrap();
    file.write_all(&WalCodec::file_header()).unwrap();
    file.write_all(&first).unwrap();
    file.write_all(&second).unwrap();
    drop(file);

    let scan = WalReader::scan_path(&path).unwrap();
    let summary = WalDiagnostics::summarize(&scan);
    assert_eq!(summary.records, 2);
    assert_eq!(summary.known_sections, 3);
    assert_eq!(summary.unknown_sections, 0);
    assert_eq!(
        summary.safe_truncate_offset,
        (16 + first.len() + second.len()) as u64
    );
    assert_eq!(summary.last_lsn, Some(16 + first.len() as u64));
}

#[test]
fn reader_stops_on_partial_tail() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("tail.aclog");
    let first = WalCodec::encode_record_at(&record(), 16).unwrap();
    let mut corrupt = WalCodec::encode_record_at(&record(), 16 + first.len() as u64).unwrap();
    corrupt.truncate(corrupt.len() - 3);
    let mut file = File::create(&path).unwrap();
    file.write_all(&WalCodec::file_header()).unwrap();
    file.write_all(&first).unwrap();
    file.write_all(&corrupt).unwrap();
    drop(file);
    let scan = WalReader::scan_path(&path).unwrap();
    assert_eq!(scan.records.len(), 1);
    assert_eq!(scan.safe_truncate_offset, (16 + first.len()) as u64);
}

#[test]
fn random_bytes_do_not_panic() {
    for size in 0..96 {
        let bytes = (0..size)
            .map(|index| ((index * 37 + size * 11) & 0xff) as u8)
            .collect::<Vec<_>>();
        let result = std::panic::catch_unwind(|| WalCodec::decode_record(&bytes));
        assert!(result.is_ok());
    }
}

#[test]
fn writer_appends_and_reader_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("writer.aclog");
    let writer = WalWriter::create(&path).unwrap();
    let ack = writer.append(record()).unwrap();
    assert_eq!(ack.durable_lsn, WalCodec::file_header_len() as u64);
    drop(writer);
    let scan = WalReader::open(&path).unwrap().scan().unwrap();
    assert_eq!(scan.records.len(), 1);
    assert_eq!(scan.records[0].record, record());
}

#[test]
fn balanced_writer_mode_appends_without_sync_requirement() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("balanced.aclog");
    let writer = WalWriter::start(&path, DurabilityMode::Balanced).unwrap();
    let ack = writer.append(record_with_payload(vec![1, 2, 3])).unwrap();
    assert_eq!(ack.durable_lsn, WalCodec::file_header_len() as u64);
}

#[test]
fn bounded_writer_queue_mode_appends_records() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bounded.aclog");
    let writer = WalWriter::start_with_options(
        &path,
        WalWriterOptions {
            durability_mode: DurabilityMode::Balanced,
            queue_capacity: Some(1),
        },
    )
    .unwrap();
    writer.append(record_with_payload(vec![1])).unwrap();
    writer.append(record_with_payload(vec![2])).unwrap();
    writer.shutdown().unwrap();

    let scan = WalReader::open(&path).unwrap().scan().unwrap();
    assert_eq!(scan.records.len(), 2);
}

#[test]
fn balanced_writer_group_commit_accepts_parallel_appends() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("group.aclog");
    let writer = WalWriter::start(&path, DurabilityMode::Balanced).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
    let handles = (0..4)
        .map(|value| {
            let writer = writer.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                writer.append(record_with_payload(vec![value])).unwrap();
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        handle.join().unwrap();
    }
    writer.shutdown().unwrap();

    let scan = WalReader::open(&path).unwrap().scan().unwrap();
    assert_eq!(scan.records.len(), 4);
}

fn rewrite_header_crc(encoded: &mut [u8]) {
    let header_len = get_u16(&encoded[4..6]) as usize;
    encoded[28..32].copy_from_slice(&0u32.to_le_bytes());
    let crc = crc32c::crc32c(&encoded[..header_len]);
    encoded[28..32].copy_from_slice(&crc.to_le_bytes());
}

fn get_u16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes(bytes.try_into().unwrap())
}

fn get_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes.try_into().unwrap())
}
