use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{
    scope_id, tokenize, Bm25Index, ClusterConfig, ConsensusState, Database, HnswIndex, LogIndex,
    NodeId, ReplicatedEntry, SearchIndexes, SearchMode, SearchQuery, Term, VectorIndex,
};

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
fn retrieve_aql_preserves_large_cell_ids_after_checkpoint_and_compact() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(u64::MAX),
            b"scope=project:investments\nstatus=ready\nlarge alpha".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(u64::MAX - 1),
            b"scope=project:investments\nstatus=draft\nlarge beta".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
        db.compact().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "large" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#,
            &view(scope_id("project:investments")),
        )
        .unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].cell_id, CellId(u64::MAX));
    assert_eq!(
        cells[0].payload,
        b"scope=project:investments\nstatus=ready\nlarge alpha"
    );
}

#[test]
fn persisted_index_overlay_removes_changed_checkpoint_candidates() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    db.patch_cell(
        CellId(1),
        b"scope=project:investments\nstatus=draft\nalpha budget".to_vec(),
    )
    .unwrap();

    let readable = view(scope_id("project:investments"));
    let ready = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#,
            &readable,
        )
        .unwrap();
    let draft = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "draft" LIMIT 10 CANDIDATES;"#,
            &readable,
        )
        .unwrap();

    assert!(ready.is_empty());
    assert_eq!(draft[0].cell_id, CellId(1));
}

#[test]
fn retrieve_aql_reports_missing_persisted_index() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    std::fs::remove_file(dir.path().join("segments").join("segment-1.acb")).unwrap();

    let result = db.retrieve_aql(
        r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#,
        &view(scope_id("project:investments")),
    );
    assert!(result.is_err());
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
fn unicode_tokenizer_handles_ru_kz_en_terms() {
    let terms = tokenize("Бюджет және Project-2025, инвестиции");
    assert!(terms.contains(&"бюджет".to_owned()));
    assert!(terms.contains(&"project".to_owned()));
    assert!(terms.contains(&"2025".to_owned()));
    assert!(terms.contains(&"инвестиции".to_owned()));
    assert!(!terms.contains(&"және".to_owned()));
}

#[test]
fn field_weighting_prioritizes_important_fields() {
    let mut index = Bm25Index::default();
    index.add_document_fields(1, &[("budget", 6), ("workflow note", 1)]);
    index.add_document_fields(2, &[("budget budget", 1)]);

    let results = index.search("budget", 2);
    assert_eq!(results[0].cell_id, 1);
}

#[test]
fn replacing_document_removes_old_postings() {
    let mut index = Bm25Index::default();
    index.add_document(1, "obsolete");
    index.add_document(1, "current");

    assert!(index.search("obsolete", 10).is_empty());
    assert_eq!(index.search("current", 10)[0].cell_id, 1);
}

#[test]
fn hybrid_search_fuses_keyword_and_vector_rankings() {
    let mut indexes = SearchIndexes::default();
    indexes.add_document(1, "ready investment budget");
    indexes.add_document(2, "workflow note");
    indexes.add_vector(1, vec![1, 0, 0]);
    indexes.add_vector(2, vec![5, 0, 0]);

    let results = indexes.search(SearchQuery {
        text: "budget",
        vector: Some(&[5, 0, 0]),
        limit: 2,
        mode: SearchMode::Hybrid,
    });
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].cell_id, 1);
    assert!(results[0].lexical_score > 0);
    assert!(results[0].vector_score > 0);
}

#[test]
fn rrf_both_lists_boosts_overlap_document() {
    let mut indexes = SearchIndexes::default();
    // Cell 1: appears in both lexical (rank 0) and vector (rank 1)
    indexes.add_document(1, "budget investment");
    indexes.add_vector(1, vec![1, 0, 0]);
    // Cell 2: appears only in lexical (rank 1)
    indexes.add_document(2, "budget workflow");
    // Cell 3: appears only in vector (rank 0)
    indexes.add_vector(3, vec![5, 0, 0]);

    let results = indexes.search(SearchQuery {
        text: "budget",
        vector: Some(&[5, 0, 0]),
        limit: 3,
        mode: SearchMode::Hybrid,
    });
    assert_eq!(results.len(), 3);
    // Cell 1 is in both lists → highest fused score
    assert_eq!(results[0].cell_id, 1);
    assert!(results[0].lexical_score > 0);
    assert!(results[0].vector_score > 0);
}

#[test]
fn rrf_empty_lexical_falls_back_to_vector_only() {
    let mut indexes = SearchIndexes::default();
    indexes.add_document(1, "unrelated text");
    indexes.add_vector(1, vec![1, 0, 0]);
    indexes.add_vector(2, vec![5, 0, 0]);

    let results = indexes.search(SearchQuery {
        text: "nonexistent_term_xyz",
        vector: Some(&[5, 0, 0]),
        limit: 2,
        mode: SearchMode::Hybrid,
    });
    assert_eq!(results.len(), 2);
    // Pure vector ranking when lexical is empty
    assert_eq!(results[0].cell_id, 2);
    assert_eq!(results[1].cell_id, 1);
    assert_eq!(results[0].lexical_score, 0);
    assert!(results[0].vector_score > 0);
}

#[test]
fn rrf_empty_vector_falls_back_to_keyword_only() {
    let mut indexes = SearchIndexes::default();
    indexes.add_document(1, "alpha budget");
    indexes.add_document(2, "beta budget");

    let results = indexes.search(SearchQuery {
        text: "budget",
        vector: Some(&[0, 0, 0]), // no vector match
        limit: 2,
        mode: SearchMode::Hybrid,
    });
    assert_eq!(results.len(), 2);
    // Pure lexical ranking when vector is empty
    assert_eq!(results[0].cell_id, 1);
    assert!(results[0].lexical_score > 0);
    assert_eq!(results[0].vector_score, 0);
}

#[test]
fn rrf_truncate_respects_limit() {
    let mut indexes = SearchIndexes::default();
    for id in 1..=10 {
        indexes.add_document(id, &format!("term {id}"));
        indexes.add_vector(id, vec![id as i16, 0]);
    }

    let results = indexes.search(SearchQuery {
        text: "term",
        vector: Some(&[5, 0]),
        limit: 3,
        mode: SearchMode::Hybrid,
    });
    assert_eq!(results.len(), 3);
}

#[test]
fn search_api_supports_keyword_and_vector_modes() {
    let mut indexes = SearchIndexes::default();
    indexes.add_document(1, "alpha budget");
    indexes.add_vector(2, vec![0, 9]);

    let keyword = indexes.search(SearchQuery {
        text: "budget",
        vector: None,
        limit: 1,
        mode: SearchMode::Keyword,
    });
    assert_eq!(keyword[0].cell_id, 1);

    let vector = indexes.search(SearchQuery {
        text: "",
        vector: Some(&[0, 2]),
        limit: 1,
        mode: SearchMode::Vector,
    });
    assert_eq!(vector[0].cell_id, 2);
}

#[test]
fn cluster_config_places_keys_on_replicas() {
    let cluster = ClusterConfig::single_node();
    let placement = cluster.placement_for_key(42).unwrap();
    assert_eq!(placement.primary.0, 1);
    assert!(cluster.owns_key(42));
}

#[test]
fn consensus_state_commits_only_after_majority_and_recovers_log() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let mut consensus = ConsensusState::new(NodeId(1), voters.clone());
    let entry = consensus.append_local(b"put cell".to_vec());
    let minority = consensus.record_acks(entry.index, BTreeSet::from([NodeId(1)]));
    assert!(!minority.committed);

    let majority = consensus.record_acks(entry.index, BTreeSet::from([NodeId(1), NodeId(2)]));
    assert!(majority.committed);
    assert_eq!(consensus.committed_entries(), vec![entry.clone()]);

    let recovered = ConsensusState::recover(
        NodeId(1),
        voters,
        vec![ReplicatedEntry {
            term: Term(2),
            index: LogIndex(1),
            payload: b"put cell".to_vec(),
        }],
        LogIndex(1),
    );
    assert_eq!(recovered.current_term, Term(2));
    assert_eq!(recovered.committed_entries().len(), 1);
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
