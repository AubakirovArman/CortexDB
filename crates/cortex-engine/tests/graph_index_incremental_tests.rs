use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::{Database, DatabaseOptions, PayloadResidency};

fn entity_cell(name: &str, kind: &str, source: &str) -> KnowledgeCell {
    let body = format!("name={name}\nkind={kind}");
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "project:investments".to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Entity,
            source: Some(source.to_owned()),
            ..KnowledgeCellMetadata::default()
        },
        body,
    )
}

fn relation_cell(subject: &str, predicate: &str, object: &str) -> KnowledgeCell {
    let body = format!("subject={subject}\npredicate={predicate}\nobject={object}");
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "project:investments".to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Relation,
            ..KnowledgeCellMetadata::default()
        },
        body,
    )
}

fn tool_cell(body: &str) -> KnowledgeCell {
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "project:investments".to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Tool,
            ..KnowledgeCellMetadata::default()
        },
        body,
    )
}

#[test]
fn knowledge_graph_incremental_store_matches_rebuild_after_mutations() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        entity_cell("Solar Plant", "project", "ifc:solar-001"),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(2),
        entity_cell("Kazakhstan", "country", "ifc:geo-001"),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(3),
        relation_cell("Solar Plant", "located_in", "Kazakhstan"),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(4),
        relation_cell("source:ifc:solar-001", "source_supports_fact", "cell:2"),
    )
    .unwrap();
    db.put_knowledge_cell(CellId(5), tool_cell("name=calculator\n\nold tool"))
        .unwrap();

    db.patch_cell(
        CellId(3),
        relation_cell("Solar Plant", "located_in", "Astana").encode_payload(),
    )
    .unwrap();
    db.patch_cell(
        CellId(5),
        tool_cell("name=planner\n\nnew tool").encode_payload(),
    )
    .unwrap();
    db.tombstone_cell(CellId(1)).unwrap();

    let incremental_index = db.knowledge_graph_index();
    let incremental_tools = db.tool_cells();
    db.close().unwrap();

    let rebuilt = Database::open(dir.path()).unwrap();
    assert_eq!(rebuilt.knowledge_graph_index(), incremental_index);
    assert_eq!(rebuilt.tool_cells(), incremental_tools);
    assert!(rebuilt.graph_entities("Solar Plant").is_empty());
    assert_eq!(rebuilt.graph_neighbors("Astana").len(), 1);
    assert_eq!(rebuilt.tool_cells()[0].name, Some("planner".to_owned()));
}

#[test]
fn lazy_knowledge_graph_queries_do_not_materialize_payloads() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_knowledge_cell(
            CellId(1),
            entity_cell("Solar Plant", "project", "ifc:solar-001"),
        )
        .unwrap();
        db.put_knowledge_cell(
            CellId(2),
            relation_cell("Solar Plant", "located_in", "Kazakhstan"),
        )
        .unwrap();
        db.put_knowledge_cell(CellId(3), tool_cell("name=calculator\n\nbudget tool"))
            .unwrap();
        db.checkpoint().unwrap();
    }

    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            payload_cache_bytes: 0,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    assert_eq!(db.storage_stats().unwrap().memtable_payload_bytes, 0);

    assert_eq!(db.graph_entities("Solar Plant").len(), 1);
    assert_eq!(db.graph_neighbors("Solar Plant").len(), 1);
    assert_eq!(db.graph_cells_for_source("ifc:solar-001"), vec![CellId(1)]);
    assert_eq!(db.tool_cells()[0].name, Some("calculator".to_owned()));
    assert_eq!(db.payload_cache_stats().segment_loads, 0);
}
