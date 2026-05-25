use std::fs::{self, File};
use std::io::Write;

use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    decode_cell_id, encode_cell_id, operation_from_decoded_wal_record, wal_record_from_operation,
    Database, DbOperation, EngineError, OperationDecoder, OperationEncoder, ReplayResult,
};
use cortex_storage::wal::{
    checksum::crc32c, SectionTag, WalCodec, WalRecord, WalRecordType, WalSection,
    WAL_RECORD_HEADER_LEN, WAL_SECTION_ENTRY_LEN,
};

#[test]
fn operation_put_cell_roundtrips_through_wal() {
    let operation = DbOperation::PutCell {
        cell_id: CellId(7),
        payload: b"hello".to_vec(),
    };
    let record = OperationEncoder::encode(&operation);
    let encoded = WalCodec::encode_record(&record).unwrap();
    let decoded = WalCodec::decode_record(&encoded).unwrap();
    assert_eq!(OperationDecoder::decode(&decoded).unwrap(), operation);
    assert_eq!(
        decode_cell_id(&encode_cell_id(CellId(7))).unwrap(),
        CellId(7)
    );
}

#[test]
fn wal_record_with_extra_unknown_section_still_decodes_operation() {
    let record = WalRecord::new(
        WalRecordType::PutCellBatch,
        vec![
            WalSection::new(SectionTag::CellCore, encode_cell_id(CellId(1))),
            WalSection::new(SectionTag::PayloadInline, b"hello".to_vec()),
            WalSection::new(SectionTag::EdgeHints, b"ignored".to_vec()),
        ],
    );
    let mut encoded = WalCodec::encode_record(&record).unwrap();
    let third_entry = WAL_RECORD_HEADER_LEN + 2 * WAL_SECTION_ENTRY_LEN;
    encoded[third_entry..third_entry + 2].copy_from_slice(&999u16.to_le_bytes());
    rewrite_header_crc(&mut encoded);
    let decoded = WalCodec::decode_record(&encoded).unwrap();
    assert!(decoded.sections.iter().any(|section| section.tag.is_none()));
    assert_eq!(
        operation_from_decoded_wal_record(&decoded).unwrap(),
        DbOperation::PutCell {
            cell_id: CellId(1),
            payload: b"hello".to_vec()
        }
    );
}

#[test]
fn invalid_operation_missing_payload_is_rejected() {
    let record = WalRecord::new(
        WalRecordType::PutCellBatch,
        vec![WalSection::new(
            SectionTag::CellCore,
            encode_cell_id(CellId(1)),
        )],
    );
    let encoded = WalCodec::encode_record(&record).unwrap();
    let decoded = WalCodec::decode_record(&encoded).unwrap();
    assert!(matches!(
        operation_from_decoded_wal_record(&decoded).unwrap_err(),
        EngineError::MissingWalSection("PayloadInline")
    ));
}

#[test]
fn put_then_get_without_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let seq = db.put_cell(CellId(1), b"hello".to_vec()).unwrap();
    assert_eq!(seq, CommitSeq(1));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"hello");
}

#[test]
fn put_then_restart_then_get() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"hello".to_vec()).unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(1));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"hello");
}

#[test]
fn multiple_cells_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"one".to_vec()).unwrap();
        db.put_cell(CellId(2), b"two".to_vec()).unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"one");
    assert_eq!(db.get_latest_cell(CellId(2)).unwrap(), b"two");
}

#[test]
fn overwrite_same_cell_latest_visible() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
    db.put_cell(CellId(1), b"v2".to_vec()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
}

#[test]
fn old_read_txn_sees_old_version() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
    let old_txn = db.read_txn();
    db.put_cell(CellId(1), b"v2".to_vec()).unwrap();
    assert_eq!(db.get_cell(old_txn, CellId(1)).unwrap(), b"v1");
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
}

#[test]
fn unknown_cell_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.get_latest_cell(CellId(404)), None);
}

#[test]
fn tombstone_hides_cell() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"hello".to_vec()).unwrap();
    db.tombstone_cell(CellId(1)).unwrap();
    assert_eq!(db.get_latest_cell(CellId(1)), None);
}

#[test]
fn tombstone_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"hello".to_vec()).unwrap();
        db.tombstone_cell(CellId(1)).unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)), None);
}

#[test]
fn patch_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
        db.patch_cell(CellId(1), b"v2".to_vec()).unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
}

#[test]
fn replay_wal_reports_records_and_safe_truncate_offset() {
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("db.aclog");
    write_header_and_records(&wal_path, &[put_record(CellId(1), b"one")], true);
    let replay = cortex_engine::replay_wal(&wal_path).unwrap();
    assert_replayed_one(replay);
}

#[test]
fn open_truncates_partial_tail_before_next_append() {
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("db.aclog");
    write_header_and_records(&wal_path, &[put_record(CellId(1), b"one")], true);
    {
        let mut db = Database::open(dir.path()).unwrap();
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"one");
        db.put_cell(CellId(2), b"two".to_vec()).unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"one");
    assert_eq!(db.get_latest_cell(CellId(2)).unwrap(), b"two");
}

#[test]
fn corrupt_payload_returns_recovery_error() {
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("db.aclog");
    write_header_and_records(&wal_path, &[put_record(CellId(1), b"one")], false);
    let mut bytes = fs::read(&wal_path).unwrap();
    *bytes.last_mut().unwrap() ^= 0xff;
    fs::write(&wal_path, bytes).unwrap();
    assert!(Database::open(dir.path()).is_err());
}

fn put_record(cell_id: CellId, payload: &[u8]) -> WalRecord {
    wal_record_from_operation(&DbOperation::PutCell {
        cell_id,
        payload: payload.to_vec(),
    })
}

fn write_header_and_records(path: &std::path::Path, records: &[WalRecord], partial_tail: bool) {
    let mut file = File::create(path).unwrap();
    file.write_all(&WalCodec::file_header()).unwrap();
    let mut lsn = WalCodec::file_header_len() as u64;
    for record in records {
        let encoded = WalCodec::encode_record_at(record, lsn).unwrap();
        lsn += encoded.len() as u64;
        file.write_all(&encoded).unwrap();
    }
    if partial_tail {
        let mut tail = WalCodec::encode_record_at(&put_record(CellId(99), b"tail"), lsn).unwrap();
        tail.truncate(tail.len() - 3);
        file.write_all(&tail).unwrap();
    }
}

fn assert_replayed_one(replay: ReplayResult) {
    assert_eq!(replay.last_seq, CommitSeq(1));
    assert_eq!(replay.records_replayed, 1);
    assert!(replay.safe_truncate_offset > WalCodec::file_header_len() as u64);
    assert_eq!(
        replay
            .memtable
            .read(
                cortex_core::memtable::ReadTxn {
                    read_seq: CommitSeq(1)
                },
                CellId(1)
            )
            .unwrap()
            .payload,
        b"one"
    );
}

fn rewrite_header_crc(encoded: &mut [u8]) {
    let header_len = u16::from_le_bytes(encoded[4..6].try_into().unwrap()) as usize;
    encoded[28..32].copy_from_slice(&0u32.to_le_bytes());
    let crc = crc32c(&encoded[..header_len]);
    encoded[28..32].copy_from_slice(&crc.to_le_bytes());
}
