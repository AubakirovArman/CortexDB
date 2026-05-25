use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, Bm25Index, ClusterConfig, Database, HnswIndex, VectorIndex};

#[test]
fn retrieve_aql_uses_engine_index_without_mock_provider() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=draft\nbeta budget".to_vec(),
    )
    .unwrap();

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#,
            &view(scope_id("project:investments")),
        )
        .unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].cell_id, CellId(1));
}

#[test]
fn bm25_and_vector_indexes_rank_candidates() {
    let mut bm25 = Bm25Index::default();
    bm25.add_document(1, "ready investment budget budget");
    bm25.add_document(2, "workflow note");
    assert_eq!(bm25.search("budget", 1)[0].cell_id, 1);

    let mut vector = VectorIndex::default();
    vector.add_vector(1, vec![3, 0, 1]);
    vector.add_vector(2, vec![0, 4, 0]);
    assert_eq!(vector.search_dot(&[2, 0, 1], 1)[0].cell_id, 1);

    let mut hnsw = HnswIndex::default();
    hnsw.add_vector(7, vec![1, 2, 3]);
    assert_eq!(hnsw.search(&[1, 1, 1], 1)[0].cell_id, 7);
}

#[test]
fn cluster_config_places_keys_on_replicas() {
    let cluster = ClusterConfig::single_node();
    let placement = cluster.placement_for_key(42).unwrap();
    assert_eq!(placement.primary.0, 1);
    assert!(cluster.owns_key(42));
}

fn view(scope: ScopeId) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
