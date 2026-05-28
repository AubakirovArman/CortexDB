use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::Database;

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

#[test]
fn graph_neighbors_finds_relations_by_subject_or_object() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        relation_cell("Solar Plant", "has_budget", "1.2B KZT"),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(2),
        relation_cell("Wind Farm", "has_budget", "800M KZT"),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(3),
        relation_cell("Solar Plant", "located_in", "Kazakhstan"),
    )
    .unwrap();

    let edges = db.graph_neighbors("Solar Plant");
    assert_eq!(edges.len(), 2);
    assert!(edges.iter().any(|e| e.predicate == "has_budget"));
    assert!(edges.iter().any(|e| e.predicate == "located_in"));

    let wind_edges = db.graph_neighbors("Wind Farm");
    assert_eq!(wind_edges.len(), 1);
    assert_eq!(wind_edges[0].predicate, "has_budget");
}

#[test]
fn graph_neighbors_ignores_non_relation_cells() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(1), relation_cell("A", "links", "B"))
        .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nplain fact cell".to_vec(),
    )
    .unwrap();

    let edges = db.graph_neighbors("A");
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].relation_cell_id, CellId(1));
}

#[test]
fn tool_cells_finds_tool_type_cells() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        KnowledgeCell::new(
            KnowledgeCellMetadata {
                scope: "project:investments".to_owned(),
                status: "ready".to_owned(),
                cell_type: KnowledgeCellType::Tool,
                ..KnowledgeCellMetadata::default()
            },
            "name=calculator\n\nA simple calculator tool.",
        ),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nplain cell".to_vec(),
    )
    .unwrap();

    let tools = db.tool_cells();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].cell_id, CellId(1));
    assert_eq!(tools[0].name, Some("calculator".to_owned()));
}
