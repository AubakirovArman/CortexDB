use std::fs::{self, File};
use std::io::Write;

use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    wal_record_from_operation_with_seq, Database, DatabaseOptions, DbOperation, RecoveryMode,
    ReplayResult,
};
use cortex_storage::wal::{DurabilityMode, WalCodec};

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
        },
    )
    .unwrap();
    assert_eq!(db.current_seq(), CommitSeq(1));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"one");
    assert_eq!(db.get_latest_cell(CellId(2)), None);
}

fn put_op(cell_id: CellId, payload: &[u8]) -> DbOperation {
    DbOperation::PutCell {
        cell_id,
        payload: payload.to_vec(),
    }
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
