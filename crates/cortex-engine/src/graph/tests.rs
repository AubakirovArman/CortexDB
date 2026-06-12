use cortex_core::CellId;

use crate::database::Database;

use super::*;

#[test]
fn persisted_knowledge_graph_survives_restart_without_rescan() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"scope=default\nstatus=ready\ntype=entity\nsource=crm\n\nname=Apollo\nkind=project"
                .to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"scope=default\nstatus=ready\ntype=relation\n\nsubject=source-1\npredicate=source_supports_fact\nobject=fact-1"
                .to_vec(),
        )
        .unwrap();
        let index = db.persist_knowledge_graph_index().unwrap();
        assert_eq!(index.entities_named("Apollo").len(), 1);
        assert_eq!(index.source_supports_fact_edges().len(), 1);
        assert!(db.knowledge_graph_index_path().exists());
    }

    let db = Database::open(dir.path()).unwrap();
    let persisted = db.read_persisted_knowledge_graph_index().unwrap().unwrap();

    assert_eq!(
        persisted.entities_named("Apollo")[0].kind.as_deref(),
        Some("project")
    );
    assert_eq!(
        persisted.source_supports_fact_edges()[0].predicate,
        "source_supports_fact"
    );
    assert_eq!(persisted.cells_for_source("crm"), vec![CellId(1)]);
}

#[test]
fn ackg_roundtrip_preserves_escaped_fields() {
    let mut index = KnowledgeGraphIndex::default();
    index.push_entity(GraphEntity {
        entity_cell_id: CellId(7),
        name: "Apollo\tProject".to_owned(),
        kind: Some("project\nteam".to_owned()),
        source_id: Some("crm".to_owned()),
    });
    index.push_edge(GraphEdge {
        relation_cell_id: CellId(8),
        subject: "Apollo\tProject".to_owned(),
        predicate: "related_to".to_owned(),
        object: "Budget\nPlan".to_owned(),
        kind: GraphEdgeKind::Other,
    });
    let decoded = KnowledgeGraphIndex::from_ackg_bytes(&index.to_ackg_bytes()).unwrap();

    assert_eq!(decoded.entities_named("Apollo\tProject").len(), 1);
    assert_eq!(decoded.neighbors("Budget\nPlan").len(), 1);
}
