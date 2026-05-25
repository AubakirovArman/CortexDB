use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cortex_storage::wal::{
    DurabilityMode, SectionTag, WalCodec, WalReader, WalRecord, WalRecordType, WalSection,
    WalWriter,
};
use cortex_storage::StorageError;

fn record() -> WalRecord {
    WalRecord::new(
        WalRecordType::PutCellBatch,
        vec![
            WalSection::new(SectionTag::CellCore, b"cell-core".to_vec()),
            WalSection::new(SectionTag::PayloadInline, b"payload".to_vec()),
        ],
    )
}

fn temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("cortexdb-{name}-{nanos}.aclog"))
}

#[test]
fn valid_record_roundtrip() {
    let encoded = WalCodec::encode_record(&record(), 42).unwrap();
    let decoded = WalCodec::decode_record(&encoded).unwrap();
    assert_eq!(decoded.lsn, 42);
    assert_eq!(decoded.record, record());
    assert_eq!(decoded.bytes_consumed, encoded.len());
}

#[test]
fn corrupt_payload_is_rejected() {
    let mut encoded = WalCodec::encode_record(&record(), 42).unwrap();
    let last = encoded.last_mut().unwrap();
    *last ^= 0xff;
    assert!(matches!(
        WalCodec::decode_record(&encoded).unwrap_err(),
        StorageError::WalChecksumMismatch
    ));
}

#[test]
fn unknown_section_tag_is_skipped() {
    let mut encoded = WalCodec::encode_record(&record(), 42).unwrap();
    encoded[32..34].copy_from_slice(&999u16.to_le_bytes());
    let mut header = encoded[..64].to_vec();
    header[28..32].copy_from_slice(&0u32.to_le_bytes());
    let crc = crc32c::crc32c(&header);
    encoded[28..32].copy_from_slice(&crc.to_le_bytes());
    let decoded = WalCodec::decode_record(&encoded).unwrap();
    assert_eq!(decoded.record.sections.len(), 1);
    assert_eq!(decoded.record.sections[0].tag, SectionTag::PayloadInline);
}

#[test]
fn reader_scans_records_and_stops_on_corrupt_tail() {
    let path = temp_path("reader");
    let first = WalCodec::encode_record(&record(), 16).unwrap();
    let mut corrupt = WalCodec::encode_record(&record(), 16 + first.len() as u64).unwrap();
    corrupt.truncate(corrupt.len() - 3);
    let mut file = File::create(&path).unwrap();
    file.write_all(&WalCodec::file_header()).unwrap();
    file.write_all(&first).unwrap();
    file.write_all(&corrupt).unwrap();
    drop(file);
    let scan = WalReader::scan(&path).unwrap();
    assert_eq!(scan.records.len(), 1);
    assert_eq!(
        scan.safe_truncate_offset,
        (WalCodec::file_header_len() + first.len()) as u64
    );
    fs::remove_file(path).unwrap();
}

#[test]
fn writer_appends_and_reader_recovers() {
    let path = temp_path("writer");
    let writer = WalWriter::start(&path, DurabilityMode::Strict).unwrap();
    let ack = writer.append(record()).unwrap();
    assert_eq!(ack.durable_lsn, WalCodec::file_header_len() as u64);
    drop(writer);
    let scan = WalReader::scan(&path).unwrap();
    assert_eq!(scan.records.len(), 1);
    assert_eq!(scan.records[0].record, record());
    fs::remove_file(path).unwrap();
}
