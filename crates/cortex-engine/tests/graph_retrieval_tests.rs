use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::Database;

fn entity_cell(name: &str, kind: &str) -> KnowledgeCell {
    let body = format!("name={name}\nkind={kind}");
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "project:investments".to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Entity,
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

#[test]
fn graph_retrieve_related_walks_multiple_hops() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(1), entity_cell("Solar Plant", "project"))
        .unwrap();
    db.put_knowledge_cell(CellId(2), entity_cell("Kazakhstan", "country"))
        .unwrap();
    db.put_knowledge_cell(CellId(3), entity_cell("Central Asia", "region"))
        .unwrap();
    db.put_knowledge_cell(
        CellId(10),
        relation_cell("Solar Plant", "located_in", "Kazakhstan"),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(11),
        relation_cell("Kazakhstan", "part_of", "Central Asia"),
    )
    .unwrap();

    let hits = db.graph_retrieve_related("Solar Plant", 2);
    let hit_ids = hits.iter().map(|hit| hit.cell_id).collect::<Vec<_>>();
    assert_eq!(
        hit_ids,
        vec![CellId(1), CellId(2), CellId(10), CellId(3), CellId(11)]
    );
}

#[test]
fn graph_retrieve_related_scores_by_proximity() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(1), entity_cell("Solar Plant", "project"))
        .unwrap();
    db.put_knowledge_cell(CellId(2), entity_cell("Kazakhstan", "country"))
        .unwrap();
    db.put_knowledge_cell(CellId(3), entity_cell("Central Asia", "region"))
        .unwrap();
    db.put_knowledge_cell(
        CellId(10),
        relation_cell("Solar Plant", "located_in", "Kazakhstan"),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(11),
        relation_cell("Kazakhstan", "part_of", "Central Asia"),
    )
    .unwrap();

    let hits = db.graph_retrieve_related("Solar Plant", 2);
    let seed = hits.iter().find(|hit| hit.cell_id == CellId(1)).unwrap();
    let first_hop = hits.iter().find(|hit| hit.cell_id == CellId(2)).unwrap();
    let second_hop = hits.iter().find(|hit| hit.cell_id == CellId(3)).unwrap();
    assert!(seed.proximity_score_q16 > first_hop.proximity_score_q16);
    assert!(first_hop.proximity_score_q16 > second_hop.proximity_score_q16);
}

#[test]
fn graph_retrieve_related_explains_edges_for_hits() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(1), entity_cell("Solar Plant", "project"))
        .unwrap();
    db.put_knowledge_cell(CellId(2), entity_cell("Kazakhstan", "country"))
        .unwrap();
    db.put_knowledge_cell(CellId(3), entity_cell("Central Asia", "region"))
        .unwrap();
    db.put_knowledge_cell(
        CellId(10),
        relation_cell("Solar Plant", "located_in", "Kazakhstan"),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(11),
        relation_cell("Kazakhstan", "part_of", "Central Asia"),
    )
    .unwrap();

    let hits = db.graph_retrieve_related("Solar Plant", 2);
    let second_hop = hits.iter().find(|hit| hit.cell_id == CellId(3)).unwrap();
    let path_ids = second_hop
        .explaining_edges
        .iter()
        .map(|edge| edge.relation_cell_id)
        .collect::<Vec<_>>();
    assert_eq!(path_ids, vec![CellId(10), CellId(11)]);
}

#[test]
fn graph_retrieve_related_respects_max_hops() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(1), entity_cell("Solar Plant", "project"))
        .unwrap();
    db.put_knowledge_cell(CellId(2), entity_cell("Kazakhstan", "country"))
        .unwrap();
    db.put_knowledge_cell(CellId(3), entity_cell("Central Asia", "region"))
        .unwrap();
    db.put_knowledge_cell(
        CellId(10),
        relation_cell("Solar Plant", "located_in", "Kazakhstan"),
    )
    .unwrap();
    db.put_knowledge_cell(
        CellId(11),
        relation_cell("Kazakhstan", "part_of", "Central Asia"),
    )
    .unwrap();

    let hits = db.graph_retrieve_related("Solar Plant", 1);
    let hit_ids = hits.iter().map(|hit| hit.cell_id).collect::<Vec<_>>();
    assert_eq!(hit_ids, vec![CellId(1), CellId(2), CellId(10)]);
}

#[test]
fn graph_retrieve_related_reports_visit_budget_exceeded() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(1), entity_cell("Hub", "project"))
        .unwrap();
    db.put_knowledge_cell(CellId(2), entity_cell("A", "project"))
        .unwrap();
    db.put_knowledge_cell(CellId(3), entity_cell("B", "project"))
        .unwrap();
    db.put_knowledge_cell(CellId(4), entity_cell("C", "project"))
        .unwrap();
    db.put_knowledge_cell(CellId(10), relation_cell("Hub", "links", "A"))
        .unwrap();
    db.put_knowledge_cell(CellId(11), relation_cell("Hub", "links", "B"))
        .unwrap();
    db.put_knowledge_cell(CellId(12), relation_cell("Hub", "links", "C"))
        .unwrap();

    let report = db.graph_retrieve_related_with_budget("Hub", 1, 2);
    let hit_ids = report
        .hits
        .iter()
        .map(|hit| hit.cell_id)
        .collect::<Vec<_>>();

    assert!(report.budget_exceeded);
    assert_eq!(report.visited_edges, 2);
    assert_eq!(
        hit_ids,
        vec![CellId(1), CellId(2), CellId(3), CellId(10), CellId(11)]
    );
}

#[test]
fn graph_retrieve_related_zero_budget_returns_seed_only() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(1), entity_cell("Hub", "project"))
        .unwrap();
    db.put_knowledge_cell(CellId(2), entity_cell("A", "project"))
        .unwrap();
    db.put_knowledge_cell(CellId(10), relation_cell("Hub", "links", "A"))
        .unwrap();

    let report = db.graph_retrieve_related_with_budget("Hub", 1, 0);
    let hit_ids = report
        .hits
        .iter()
        .map(|hit| hit.cell_id)
        .collect::<Vec<_>>();

    assert!(report.budget_exceeded);
    assert_eq!(report.visited_edges, 0);
    assert_eq!(hit_ids, vec![CellId(1)]);
}
