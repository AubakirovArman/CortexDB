use std::fs::{self, File};
use std::io::Write;

use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    wal_record_from_operation_with_seq, Database, DatabaseOptions, DbOperation, RecoveryMode,
    ReplayResult,
};
use cortex_storage::wal::{
    DurabilityMode, SectionTag, WalCodec, WalRecord, WalRecordType, WalSection,
};

#[test]
fn replay_wal_reports_records_and_safe_truncate_offset() {
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("db.aclog");
    write_header_and_records(
        &wal_path,
        &[(CommitSeq(9), put_op(CellId(1), b"one"))],
        true,
    );
    let replay = cortex_engine::replay_wal(&wal_path).unwrap();
    assert_replayed_one(replay, CommitSeq(9));
}

#[test]
fn replay_wal_reports_metrics_for_seen_applied_and_skipped_records() {
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("db.aclog");
    write_header_and_records(
        &wal_path,
        &[
            (CommitSeq(1), put_op(CellId(1), b"one")),
            (CommitSeq(2), put_op(CellId(2), b"two")),
        ],
        false,
    );

    let replay =
        cortex_engine::replay_wal_into(&wal_path, Default::default(), CommitSeq(1)).unwrap();
    assert_eq!(replay.metrics.records_seen, 2);
    assert_eq!(replay.metrics.records_applied, 1);
    assert_eq!(replay.metrics.records_skipped, 1);
    assert!(replay.metrics.payload_bytes > 0);
    assert_eq!(
        replay.metrics.safe_truncate_offset,
        replay.safe_truncate_offset
    );
}

#[test]
fn open_truncates_partial_tail_before_next_append() {
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("db.aclog");
    write_header_and_records(
        &wal_path,
        &[(CommitSeq(1), put_op(CellId(1), b"one"))],
        true,
    );
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
    write_header_and_records(
        &wal_path,
        &[(CommitSeq(1), put_op(CellId(1), b"one"))],
        false,
    );
    corrupt_last_byte(&wal_path);
    assert!(Database::open(dir.path()).is_err());
}

#[test]
fn best_effort_recovery_stops_at_corrupt_payload() {
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("db.aclog");
    write_header_and_records(
        &wal_path,
        &[
            (CommitSeq(1), put_op(CellId(1), b"one")),
            (CommitSeq(2), put_op(CellId(2), b"two")),
        ],
        false,
    );
    corrupt_last_byte(&wal_path);
    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            durability_mode: DurabilityMode::Strict,
            recovery_mode: RecoveryMode::BestEffort,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    assert_eq!(db.current_seq(), CommitSeq(1));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"one");
    assert_eq!(db.get_latest_cell(CellId(2)), None);
}

#[test]
fn replay_wal_is_idempotent_when_replayed_from_last_seq() {
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("db.aclog");
    write_header_and_records(
        &wal_path,
        &[
            (CommitSeq(1), put_op(CellId(1), b"one")),
            (CommitSeq(2), put_op(CellId(2), b"two")),
        ],
        false,
    );

    let first = cortex_engine::replay_wal(&wal_path).unwrap();
    let second =
        cortex_engine::replay_wal_into(&wal_path, first.memtable.clone(), first.last_seq).unwrap();

    assert_eq!(second.records_replayed, 0);
    assert_eq!(second.metrics.records_skipped, 2);
    assert_eq!(second.last_seq, CommitSeq(2));
}

#[test]
fn replay_error_does_not_mutate_caller_memtable() {
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("db.aclog");
    write_invalid_record_missing_payload(&wal_path);
    let mut base = cortex_core::memtable::MemTable::default();
    base.put_cell(CellId(7), CommitSeq(7), b"base".to_vec());

    let error = cortex_engine::replay_wal_into(&wal_path, base.clone(), CommitSeq(7))
        .unwrap_err()
        .to_string();

    assert!(error.contains("missing WAL section"));
    let txn = cortex_core::memtable::ReadTxn {
        read_seq: CommitSeq(7),
    };
    assert_eq!(base.read(txn, CellId(7)).unwrap().payload, b"base");
}

fn put_op(cell_id: CellId, payload: &[u8]) -> DbOperation {
    DbOperation::PutCell {
        cell_id,
        payload: payload.to_vec(),
    }
}

fn write_invalid_record_missing_payload(path: &std::path::Path) {
    let mut file = File::create(path).unwrap();
    file.write_all(&WalCodec::file_header()).unwrap();
    let record = WalRecord::new(
        WalRecordType::PutCellBatch,
        vec![WalSection::new(
            SectionTag::CellCore,
            cortex_engine::encode_cell_core(CellId(1), CommitSeq(8)),
        )],
    );
    let encoded = WalCodec::encode_record_at(&record, WalCodec::file_header_len() as u64).unwrap();
    file.write_all(&encoded).unwrap();
}

fn write_header_and_records(
    path: &std::path::Path,
    records: &[(CommitSeq, DbOperation)],
    partial_tail: bool,
) {
    let mut file = File::create(path).unwrap();
    file.write_all(&WalCodec::file_header()).unwrap();
    let mut lsn = WalCodec::file_header_len() as u64;
    for (seq, operation) in records {
        let record = wal_record_from_operation_with_seq(*seq, operation);
        let encoded = WalCodec::encode_record_at(&record, lsn).unwrap();
        lsn += encoded.len() as u64;
        file.write_all(&encoded).unwrap();
    }
    if partial_tail {
        let tail_record =
            wal_record_from_operation_with_seq(CommitSeq(99), &put_op(CellId(99), b"tail"));
        let mut tail = WalCodec::encode_record_at(&tail_record, lsn).unwrap();
        tail.truncate(tail.len() - 3);
        file.write_all(&tail).unwrap();
    }
}

fn assert_replayed_one(replay: ReplayResult, seq: CommitSeq) {
    assert_eq!(replay.last_seq, seq);
    assert_eq!(replay.records_replayed, 1);
    assert_eq!(replay.metrics.records_applied, 1);
    assert_eq!(replay.metrics.records_seen, 1);
    assert!(replay.safe_truncate_offset > WalCodec::file_header_len() as u64);
    assert_eq!(
        replay
            .memtable
            .read(cortex_core::memtable::ReadTxn { read_seq: seq }, CellId(1))
            .unwrap()
            .payload,
        b"one"
    );
}

fn corrupt_last_byte(path: &std::path::Path) {
    let mut bytes = fs::read(path).unwrap();
    *bytes.last_mut().unwrap() ^= 0xff;
    fs::write(path, bytes).unwrap();
}
