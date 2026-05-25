use std::fs::File;
use std::io::Write;

use cortex_core::CellId;
use cortex_engine::{encode_cell_id, replay_wal, EngineError};
use cortex_storage::wal::{SectionTag, WalCodec, WalRecord, WalRecordType, WalSection};

#[test]
fn replay_wal_rejects_operation_without_commit_seq() {
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("db.aclog");

    let record = WalRecord::new(
        WalRecordType::PutCellBatch,
        vec![
            WalSection::new(SectionTag::CellCore, encode_cell_id(CellId(1))),
            WalSection::new(SectionTag::PayloadInline, b"hello".to_vec()),
        ],
    );

    let mut file = File::create(&wal_path).unwrap();
    file.write_all(&WalCodec::file_header()).unwrap();
    let encoded = WalCodec::encode_record_at(&record, WalCodec::file_header_len() as u64).unwrap();
    file.write_all(&encoded).unwrap();

    assert!(matches!(
        replay_wal(&wal_path).unwrap_err(),
        EngineError::MissingCommitSeq
    ));
}
