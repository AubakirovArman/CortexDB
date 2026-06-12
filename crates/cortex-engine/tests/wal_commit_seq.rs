use std::fs::File;
use std::io::Write;

use cortex_core::memtable::ReadTxn;
use cortex_core::{CellDescriptor, CellId, CommitSeq, KnowledgeCellType};
use cortex_engine::{encode_cell_core, encode_cell_id, replay_wal, EngineError};
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

#[test]
fn replay_wal_uses_binary_cell_descriptor_section_when_present() {
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("db.aclog");
    let descriptor = CellDescriptor {
        scope: "project:typed".to_owned(),
        status: "verified".to_owned(),
        cell_type: KnowledgeCellType::Fact,
        source_trust_q16: Some(60_000),
        ..CellDescriptor::default()
    };

    let record = WalRecord::new(
        WalRecordType::PutCellBatch,
        vec![
            WalSection::new(
                SectionTag::CellCore,
                encode_cell_core(CellId(1), CommitSeq(1)),
            ),
            WalSection::new(
                SectionTag::PayloadInline,
                b"scope=project:payload\nstatus=ready\ntype=raw\nsource_trust_q16=1000\n\nhello"
                    .to_vec(),
            ),
            WalSection::new(SectionTag::CellDescriptor, descriptor.encode_section_v1()),
        ],
    );

    let mut file = File::create(&wal_path).unwrap();
    file.write_all(&WalCodec::file_header()).unwrap();
    let encoded = WalCodec::encode_record_at(&record, WalCodec::file_header_len() as u64).unwrap();
    file.write_all(&encoded).unwrap();

    let replay = replay_wal(&wal_path).unwrap();
    let version = replay
        .memtable
        .read(ReadTxn::at(CommitSeq(1)), CellId(1))
        .unwrap();
    assert_eq!(version.descriptor, descriptor);
    assert_eq!(
        version.payload,
        b"scope=project:payload\nstatus=ready\ntype=raw\nsource_trust_q16=1000\n\nhello"
    );
}
