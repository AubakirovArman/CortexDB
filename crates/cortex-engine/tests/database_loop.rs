use cortex_core::{CellDescriptor, CellId, CommitSeq, KnowledgeCellType};
use cortex_engine::{
    decode_cell_core, decode_cell_id, descriptor_from_decoded_wal_record, encode_cell_core,
    encode_cell_id, operation_from_decoded_wal_record, Database, DbOperation, EngineError,
    OperationDecoder, OperationEncoder, WriteBatch,
};
use cortex_storage::wal::{
    checksum::crc32c, SectionTag, WalCodec, WalReader, WalRecord, WalRecordType, WalSection,
    WAL_RECORD_HEADER_LEN, WAL_SECTION_ENTRY_LEN,
};

#[test]
fn operation_put_cell_roundtrips_through_wal() {
    let operation = DbOperation::PutCell {
        cell_id: CellId(7),
        payload: b"scope=project:test\nstatus=ready\ntype=fact\nsource_trust_q16=50000\n\nhello"
            .to_vec(),
    };
    let record = OperationEncoder::encode_with_seq(CommitSeq(3), &operation);
    let encoded = WalCodec::encode_record(&record).unwrap();
    let decoded = WalCodec::decode_record(&encoded).unwrap();
    assert_eq!(OperationDecoder::decode(&decoded).unwrap(), operation);
    assert_eq!(
        OperationDecoder::decode_with_seq(&decoded).unwrap().seq,
        CommitSeq(3)
    );
    assert_eq!(
        decode_cell_core(&decoded.sections[0].data).unwrap().seq,
        Some(CommitSeq(3))
    );
    assert_eq!(
        decode_cell_id(&encode_cell_id(CellId(7))).unwrap(),
        CellId(7)
    );
    let descriptor = descriptor_from_decoded_wal_record(&decoded)
        .unwrap()
        .expect("descriptor section");
    assert_eq!(
        descriptor,
        CellDescriptor {
            scope: "project:test".to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Fact,
            source_trust_q16: Some(50_000),
            ..CellDescriptor::default()
        }
    );
}

#[test]
fn operation_encoder_without_commit_seq_is_rejected() {
    let operation = DbOperation::PutCell {
        cell_id: CellId(7),
        payload: b"hello".to_vec(),
    };
    assert!(matches!(
        OperationEncoder::encode(&operation).unwrap_err(),
        EngineError::MissingCommitSeq
    ));
}

#[test]
fn operation_decoder_rejects_missing_commit_seq() {
    let record = WalRecord::new(
        WalRecordType::PutCellBatch,
        vec![
            WalSection::new(SectionTag::CellCore, encode_cell_id(CellId(1))),
            WalSection::new(SectionTag::PayloadInline, b"hello".to_vec()),
        ],
    );
    let encoded = WalCodec::encode_record(&record).unwrap();
    let decoded = WalCodec::decode_record(&encoded).unwrap();
    assert!(matches!(
        operation_from_decoded_wal_record(&decoded).unwrap_err(),
        EngineError::MissingCommitSeq
    ));
}

#[test]
fn wal_record_with_extra_unknown_section_still_decodes_operation() {
    let record = WalRecord::new(
        WalRecordType::PutCellBatch,
        vec![
            WalSection::new(
                SectionTag::CellCore,
                encode_cell_core(CellId(1), CommitSeq(1)),
            ),
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
            encode_cell_core(CellId(1), CommitSeq(1)),
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
fn test_recovery_with_multiple_rotated_wal_files() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"val1".to_vec()).unwrap();
        db.put_cell(CellId(2), b"val2".to_vec()).unwrap();
    }

    let active_wal = dir.path().join("db.aclog");
    assert!(active_wal.exists());
    let rotated_wal = dir.path().join("db.100.aclog");
    std::fs::rename(&active_wal, &rotated_wal).unwrap();

    {
        let mut db = Database::open(dir.path()).unwrap();
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"val1".to_vec());
        assert_eq!(db.get_latest_cell(CellId(2)).unwrap(), b"val2".to_vec());

        db.put_cell(CellId(3), b"val3".to_vec()).unwrap();
    }

    {
        let db = Database::open(dir.path()).unwrap();
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"val1".to_vec());
        assert_eq!(db.get_latest_cell(CellId(2)).unwrap(), b"val2".to_vec());
        assert_eq!(db.get_latest_cell(CellId(3)).unwrap(), b"val3".to_vec());
    }
}

#[test]
fn test_put_cells_batch_put() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cells(vec![
            (CellId(1), b"batch1".to_vec()),
            (CellId(2), b"batch2".to_vec()),
        ])
        .unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"batch1");
    assert_eq!(db.get_latest_cell(CellId(2)).unwrap(), b"batch2");
}

#[test]
fn write_batch_mixes_put_patch_and_tombstone_atomically() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"old".to_vec()).unwrap();
        let seq = db
            .write_batch(
                WriteBatch::new()
                    .patch_cell(CellId(1), b"new".to_vec())
                    .put_cell(CellId(2), b"temporary".to_vec())
                    .tombstone_cell(CellId(2)),
            )
            .unwrap();
        assert_eq!(seq, CommitSeq(4));
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"new");
        assert!(db.get_latest_cell(CellId(2)).is_none());
    }

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(4));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"new");
    assert!(db.get_latest_cell(CellId(2)).is_none());
}

#[test]
fn write_batch_validation_error_happens_before_wal_append() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"before".to_vec()).unwrap();
        let error = db
            .write_batch(
                WriteBatch::new()
                    .patch_cell(CellId(1), b"after".to_vec())
                    .tombstone_cell(CellId(99)),
            )
            .unwrap_err();
        assert!(matches!(error, EngineError::Core(_)));
        assert_eq!(db.current_seq(), CommitSeq(1));
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"before");
    }

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(1));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"before");
    assert!(db.get_latest_cell(CellId(99)).is_none());
}

#[test]
fn incomplete_write_batch_is_not_replayed_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let wal_path = dir.path().join("db.aclog");
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"before".to_vec()).unwrap();
        db.write_batch(
            WriteBatch::new()
                .patch_cell(CellId(1), b"after".to_vec())
                .put_cell(CellId(2), b"new".to_vec()),
        )
        .unwrap();
        db.close().unwrap();
    }

    let scan = WalReader::scan_path(&wal_path).unwrap();
    let commit_lsn = scan
        .records
        .iter()
        .find(|record| record.record.record_type == WalRecordType::WriteBatchCommit)
        .expect("write batch commit marker should be present")
        .lsn;
    std::fs::OpenOptions::new()
        .write(true)
        .open(&wal_path)
        .unwrap()
        .set_len(commit_lsn)
        .unwrap();

    {
        let db = Database::open(dir.path()).unwrap();
        assert_eq!(db.current_seq(), CommitSeq(1));
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"before");
        assert!(db.get_latest_cell(CellId(2)).is_none());
    }

    let scan_after_reopen = WalReader::scan_path(&wal_path).unwrap();
    assert!(scan_after_reopen
        .records
        .iter()
        .all(|record| record.record.record_type != WalRecordType::WriteBatchBegin));
}

fn rewrite_header_crc(encoded: &mut [u8]) {
    let header_len = u16::from_le_bytes(encoded[4..6].try_into().unwrap()) as usize;
    encoded[28..32].copy_from_slice(&0u32.to_le_bytes());
    let crc = crc32c(&encoded[..header_len]);
    encoded[28..32].copy_from_slice(&crc.to_le_bytes());
}
