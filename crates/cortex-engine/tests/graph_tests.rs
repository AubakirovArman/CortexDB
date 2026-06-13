use cortex_core::memtable::CellVersion;
use cortex_core::{
    CellDescriptor, CellId, CommitSeq, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType,
};
use cortex_engine::graph::KnowledgeGraphIndex;
use cortex_engine::{Database, GraphEdgeKind};

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
fn knowledge_graph_index_uses_descriptor_type_over_payload_type() {
    let payload = b"scope=project:investments\nstatus=ready\ntype=relation\nsource=payload-source\n\nsubject=A\npredicate=links\nobject=B"
        .to_vec();
    let descriptor = CellDescriptor {
        scope: "project:investments".to_owned(),
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::Raw,
        source: Some("descriptor-source".to_owned()),
        ..CellDescriptor::default()
    };
    let version =
        CellVersion::new_with_descriptor(CellId(99), CommitSeq(1), payload, 0, descriptor);

    let index = KnowledgeGraphIndex::from_versions(vec![version]);

    assert!(index.neighbors("A").is_empty());
    assert_eq!(
        index.cells_for_source("payload-source"),
        Vec::<CellId>::new()
    );
    assert_eq!(
        index.cells_for_source("descriptor-source"),
        vec![CellId(99)]
    );
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

#[test]
fn tool_cells_allow_missing_name_without_panicking() {
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
            "A tool cell without an explicit name.",
        ),
    )
    .unwrap();

    let tools = db.tool_cells();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].cell_id, CellId(1));
    assert_eq!(tools[0].name, None);
}

#[test]
fn knowledge_graph_index_groups_entities_edges_and_sources() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        entity_cell("Solar Plant", "project", "wb:solar-001"),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(2),
        entity_cell("Kazakhstan", "country", "wb:solar-001"),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(3),
        relation_cell("Solar Plant", "located_in", "Kazakhstan"),
    )
    .unwrap();

    let index = db.knowledge_graph_index();
    let entities = index.entities_named("Solar Plant");
    assert_eq!(entities.len(), 1);
    assert_eq!(entities[0].kind, Some("project".to_owned()));

    let edges = index.neighbors("Kazakhstan");
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].subject, "Solar Plant");
    assert_eq!(edges[0].predicate, "located_in");

    let source_cells = index.cells_for_source("wb:solar-001");
    assert_eq!(source_cells, vec![CellId(1), CellId(2)]);
}

#[test]
fn knowledge_graph_indexes_source_supports_fact_edges() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        relation_cell("source:ifc:solar-001", "source_supports_fact", "cell:42"),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(2),
        relation_cell("source:ifc:wind-001", "located_in", "Kazakhstan"),
    )
    .unwrap();

    let edges = db.graph_source_supports_fact_edges();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].relation_cell_id, CellId(1));
    assert_eq!(edges[0].subject, "source:ifc:solar-001");
    assert_eq!(edges[0].predicate, "source_supports_fact");
    assert_eq!(edges[0].object, "cell:42");
    assert_eq!(edges[0].kind, GraphEdgeKind::SourceSupportsFact);
}

#[test]
fn knowledge_graph_indexes_fact_contradicts_fact_edges() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        relation_cell("cell:42", "fact_contradicts_fact", "cell:43"),
    )
    .unwrap();
    db.put_knowledge_cell(CellId(2), relation_cell("cell:44", "supports", "cell:45"))
        .unwrap();

    let edges = db.graph_fact_contradicts_fact_edges();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].relation_cell_id, CellId(1));
    assert_eq!(edges[0].subject, "cell:42");
    assert_eq!(edges[0].predicate, "fact_contradicts_fact");
    assert_eq!(edges[0].object, "cell:43");
    assert_eq!(edges[0].kind, GraphEdgeKind::FactContradictsFact);
}

#[test]
fn graph_edge_kind_normalizes_predicate_aliases() {
    assert_eq!(
        GraphEdgeKind::from_predicate("source-supports-fact"),
        GraphEdgeKind::SourceSupportsFact
    );
    assert_eq!(
        GraphEdgeKind::from_predicate(" CONTRADICTS "),
        GraphEdgeKind::FactContradictsFact
    );
    assert_eq!(
        GraphEdgeKind::from_predicate("located_in"),
        GraphEdgeKind::Other
    );
}

#[test]
fn knowledge_graph_index_survives_checkpoint_and_reopen() {
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
            relation_cell("Solar Plant", "has_sector", "renewable_energy"),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.graph_entities("Solar Plant").len(), 1);
    assert_eq!(db.graph_neighbors("Solar Plant").len(), 1);
    assert_eq!(db.graph_cells_for_source("ifc:solar-001"), vec![CellId(1)]);
}

#[test]
fn knowledge_graph_store_tracks_patch_tombstone_checkpoint_and_reopen() {
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
            relation_cell("Solar Plant", "has_sector", "renewable_energy"),
        )
        .unwrap();
        db.put_knowledge_cell(CellId(3), tool_cell("name=calculator\n\nold tool"))
            .unwrap();

        db.patch_cell(
            CellId(2),
            relation_cell("Wind Farm", "has_sector", "renewable_energy").encode_payload(),
        )
        .unwrap();
        db.patch_cell(
            CellId(3),
            tool_cell("name=updated\n\nnew tool").encode_payload(),
        )
        .unwrap();
        db.tombstone_cell(CellId(1)).unwrap();

        assert!(db.graph_entities("Solar Plant").is_empty());
        assert!(db.graph_neighbors("Solar Plant").is_empty());
        assert_eq!(db.graph_neighbors("Wind Farm").len(), 1);
        assert!(db.graph_cells_for_source("ifc:solar-001").is_empty());
        assert_eq!(db.tool_cells()[0].name, Some("updated".to_owned()));
        db.checkpoint().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    assert!(db.graph_entities("Solar Plant").is_empty());
    assert!(db.graph_neighbors("Solar Plant").is_empty());
    assert_eq!(db.graph_neighbors("Wind Farm").len(), 1);
    assert!(db.graph_cells_for_source("ifc:solar-001").is_empty());
    assert_eq!(db.tool_cells()[0].name, Some("updated".to_owned()));
}
