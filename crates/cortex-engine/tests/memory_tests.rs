use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::Database;

#[test]
fn expired_memory_cells_reports_only_due_memory() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(1), memory_cell(100, Some(10)))
        .unwrap();
    db.put_knowledge_cell(CellId(2), memory_cell(100, Some(200)))
        .unwrap();
    db.put_knowledge_cell(CellId(3), memory_cell(100, None))
        .unwrap();

    let expired = db.expired_memory_cells(111);
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].cell_id, CellId(1));
    assert_eq!(expired[0].expired_at_unix_seconds, 110);
}

#[test]
fn expire_memory_cells_tombstones_through_wal_and_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_knowledge_cell(CellId(1), memory_cell(100, Some(10)))
            .unwrap();
        let expired = db.expire_memory_cells(111).unwrap();
        assert_eq!(expired[0].cell_id, CellId(1));
        assert!(db.get_latest_cell(CellId(1)).is_none());
    }

    let db = Database::open(dir.path()).unwrap();
    assert!(db.get_latest_cell(CellId(1)).is_none());
}

fn memory_cell(created: u64, ttl: Option<u64>) -> KnowledgeCell {
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "project:investments".to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Memory,
            memory_type: Some("decision".to_owned()),
            ttl_seconds: ttl,
            created_unix_seconds: Some(created),
            source: Some("test".to_owned()),
        },
        "memory payload",
    )
}
