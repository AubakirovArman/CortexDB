use std::sync::Arc;

use cortex_core::memtable::ReadTxn;
use cortex_core::{
    CellDescriptor, CellId, CommitSeq, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType,
};
use cortex_storage::manifest::{ManifestSegment, StorageManifest};
use cortex_storage::segment::{SegmentCellRef, SegmentWriter};

use super::{load_checkpoint, segment_path, segments_path};
use crate::options::PayloadResidency;
use crate::Database;

fn cell(body: &str) -> KnowledgeCell {
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "project:cache".to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Fact,
            memory_type: None,
            ttl_seconds: None,
            created_unix_seconds: None,
            source_trust_q16: None,
            source: Some("cache-test".to_owned()),
        },
        body,
    )
}

#[test]
fn reopened_checkpointed_db_reuses_persisted_index_state_across_calls() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_knowledge_cell(CellId(1), cell("solar plant budget approved"))
            .unwrap();
        db.put_knowledge_cell(CellId(2), cell("wind farm budget approved"))
            .unwrap();
        db.checkpoint().unwrap();
    }

    // Reopen: WAL is truncated and the memtable is empty, so searches take
    // the persisted fast path. The decoded index must be cached, not rebuilt
    // from the multi-gigabyte segment files on every call.
    let db = Database::open(dir.path()).unwrap();
    let first = db.persisted_index_state_cached().unwrap();
    let second = db.persisted_index_state_cached().unwrap();
    assert!(
        Arc::ptr_eq(&first, &second),
        "second call must reuse the cached persisted index, not rebuild it"
    );
    assert!(!first.candidate_to_cell.is_empty());
}

#[test]
fn checkpoint_that_changes_segments_invalidates_cached_persisted_index() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(1), cell("solar plant budget approved"))
        .unwrap();
    db.checkpoint().unwrap();
    let before = db.persisted_index_state_cached().unwrap();

    // A new checkpoint that writes another segment changes the live-segment
    // fingerprint, so the cache key no longer matches and the state is rebuilt.
    db.put_knowledge_cell(CellId(2), cell("wind farm budget approved"))
        .unwrap();
    db.checkpoint().unwrap();
    let after = db.persisted_index_state_cached().unwrap();

    assert!(
        !Arc::ptr_eq(&before, &after),
        "a checkpoint that rewrites segments must invalidate the cached index"
    );
    assert_eq!(after.candidate_to_cell.len(), 2);
}

#[test]
fn load_checkpoint_prefers_segment_descriptor_over_payload_headers() {
    let dir = tempfile::tempdir().unwrap();
    let segments = segments_path(dir.path());
    std::fs::create_dir_all(&segments).unwrap();
    let payload = b"scope=project:payload\nstatus=draft\ntype=raw\nsource_trust_q16=1000\n\nhello";
    let descriptor = CellDescriptor {
        scope: "project:typed".to_owned(),
        status: "verified".to_owned(),
        cell_type: KnowledgeCellType::Fact,
        memory_type: None,
        ttl_seconds: None,
        created_unix_seconds: Some(123),
        source_trust_q16: Some(60_000),
        source: Some("segment-descriptor".to_owned()),
        citation: None,
        content_hash: None,
        parent_id: None,
        valid_from: None,
        valid_to: None,
    };
    let cells = [SegmentCellRef {
        candidate_id: 1,
        cell_id: 42,
        created_seq: 7,
        deleted_seq: None,
        descriptor: Some(descriptor.encode_section_v1()),
        payload,
    }];
    SegmentWriter::write_refs(segment_path(&segments, 1), &cells).unwrap();
    StorageManifest {
        generation: 1,
        checkpoint_seq: 7,
        live_segments: vec![ManifestSegment {
            id: 1,
            generation: 1,
            checkpoint_seq: 7,
            cell_count: 1,
        }],
        ..StorageManifest::default()
    }
    .store(super::manifest_path(dir.path()))
    .unwrap();

    let checkpoint = load_checkpoint(dir.path(), PayloadResidency::Memory).unwrap();
    let version = checkpoint
        .memtable
        .read(ReadTxn::at(CommitSeq(7)), CellId(42))
        .unwrap();

    assert_eq!(version.payload, payload);
    assert_eq!(version.descriptor, descriptor);
}
